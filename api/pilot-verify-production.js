const crypto = require("node:crypto");

const VERIFY_TIMEOUT_MS = 8_000;
const EXPECTED_OWNER_ID = "team_jC9jlJ9GZ9GSjrbYoD0pin3U";
const EXPECTED_SOURCE_PROJECT = "routalk-pilot";
const EXPECTED_SOURCE_ENVIRONMENT = "production";
const OIDC_ISSUER = "https://oidc.vercel.com/routalk-builder";
const OIDC_AUDIENCE = "https://vercel.com/routalk-builder";
const OIDC_JWKS = new URL(`${OIDC_ISSUER}/.well-known/jwks`);

function setSecurityHeaders(response, requestId) {
  response.setHeader("Cache-Control", "no-store, max-age=0");
  response.setHeader("Pragma", "no-cache");
  response.setHeader("Referrer-Policy", "no-referrer");
  response.setHeader("X-Content-Type-Options", "nosniff");
  response.setHeader("X-Frame-Options", "DENY");
  response.setHeader("X-Pilot-Request-Id", requestId);
}

function validSha(value) {
  return /^[a-f0-9]{40}$/i.test(value || "");
}

function base64urlToBuffer(value) {
  const normalized = value.replace(/-/g, "+").replace(/_/g, "/");
  const padding = "=".repeat((4 - (normalized.length % 4)) % 4);
  return Buffer.from(normalized + padding, "base64");
}

function parseJwt(token) {
  const parts = String(token || "").split(".");
  if (parts.length !== 3) throw new Error("INVALID_TOKEN");
  const header = JSON.parse(base64urlToBuffer(parts[0]).toString("utf8"));
  const payload = JSON.parse(base64urlToBuffer(parts[1]).toString("utf8"));
  return {
    header,
    payload,
    signingInput: `${parts[0]}.${parts[1]}`,
    signature: base64urlToBuffer(parts[2]),
  };
}

async function fetchSigningKey(kid) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), VERIFY_TIMEOUT_MS);
  try {
    const result = await fetch(OIDC_JWKS, {
      headers: { Accept: "application/json" },
      cache: "no-store",
      signal: controller.signal,
    });
    if (!result.ok) throw new Error("JWKS_UNAVAILABLE");
    const data = await result.json();
    const jwk = Array.isArray(data.keys)
      ? data.keys.find((key) => key && key.kid === kid && key.kty === "RSA")
      : null;
    if (!jwk) throw new Error("SIGNING_KEY_NOT_FOUND");
    return crypto.createPublicKey({ key: jwk, format: "jwk" });
  } finally {
    clearTimeout(timeout);
  }
}

async function verifyPilotIdentity(token) {
  const parsed = parseJwt(token);
  if (parsed.header.alg !== "RS256" || typeof parsed.header.kid !== "string") {
    throw new Error("INVALID_TOKEN_HEADER");
  }

  const key = await fetchSigningKey(parsed.header.kid);
  const validSignature = crypto.verify(
    "RSA-SHA256",
    Buffer.from(parsed.signingInput),
    key,
    parsed.signature,
  );
  if (!validSignature) throw new Error("INVALID_TOKEN_SIGNATURE");

  const now = Math.floor(Date.now() / 1000);
  const payload = parsed.payload;
  const expectedSubject =
    `owner:routalk-builder:project:${EXPECTED_SOURCE_PROJECT}:environment:${EXPECTED_SOURCE_ENVIRONMENT}`;

  if (
    payload.iss !== OIDC_ISSUER ||
    payload.aud !== OIDC_AUDIENCE ||
    payload.owner_id !== EXPECTED_OWNER_ID ||
    payload.project !== EXPECTED_SOURCE_PROJECT ||
    payload.environment !== EXPECTED_SOURCE_ENVIRONMENT ||
    payload.sub !== expectedSubject ||
    typeof payload.exp !== "number" ||
    typeof payload.iat !== "number" ||
    payload.exp <= now ||
    payload.iat > now + 60
  ) {
    throw new Error("INVALID_TOKEN_CLAIMS");
  }

  return payload;
}

function requestBearer(request) {
  const authorization = request.headers.authorization || "";
  return authorization.startsWith("Bearer ") ? authorization.slice(7).trim() : "";
}

module.exports = async function handler(request, response) {
  const requestId = crypto.randomUUID();
  setSecurityHeaders(response, requestId);

  if (request.method !== "GET") {
    response.setHeader("Allow", "GET");
    return response.status(405).json({
      error: "Method not allowed",
      request_id: requestId,
    });
  }

  const expectedRevisionValue = Array.isArray(request.query.expected_revision)
    ? request.query.expected_revision[0]
    : request.query.expected_revision;
  const expectedRevision =
    typeof expectedRevisionValue === "string" ? expectedRevisionValue.trim() : "";

  if (!validSha(expectedRevision)) {
    return response.status(400).json({
      state: "FAILED",
      error: "Invalid expected_revision",
      request_id: requestId,
    });
  }

  try {
    await verifyPilotIdentity(requestBearer(request));
  } catch {
    return response.status(401).json({
      error: "Unauthorized",
      request_id: requestId,
    });
  }

  const observedRevision = process.env.VERCEL_GIT_COMMIT_SHA || "";
  const revisionMatch =
    validSha(observedRevision) &&
    observedRevision.toLowerCase() === expectedRevision.toLowerCase();

  return response.status(200).json({
    state: revisionMatch ? "READY" : "WAITING_FOR_REVISION",
    expected_revision: expectedRevision,
    observed_revision: validSha(observedRevision) ? observedRevision : null,
    revision_match: revisionMatch,
    health_ready: validSha(observedRevision),
    verified_at: new Date().toISOString(),
    request_id: requestId,
  });
};

module.exports._test = {
  parseJwt,
  validSha,
  verifyPilotIdentity,
};
