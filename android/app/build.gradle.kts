import java.util.Base64

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

val scoutKeystoreBase64 = System.getenv("SCOUT_SIGNING_KEYSTORE_B64")
val scoutStorePassword = System.getenv("SCOUT_SIGNING_STORE_PASSWORD")
val scoutKeyAlias = System.getenv("SCOUT_SIGNING_KEY_ALIAS")
val scoutKeyPassword = System.getenv("SCOUT_SIGNING_KEY_PASSWORD")

val scoutVersionCode =
    System.getenv("SCOUT_VERSION_CODE")
        ?.toIntOrNull()
        ?.takeIf { it > 2 }
        ?: 2

val scoutVersionName =
    System.getenv("SCOUT_VERSION_NAME")
        ?.takeIf { it.isNotBlank() }
        ?: "0.2.0"

val scoutSigningAvailable =
    !scoutKeystoreBase64.isNullOrBlank() &&
        !scoutStorePassword.isNullOrBlank() &&
        !scoutKeyAlias.isNullOrBlank() &&
        !scoutKeyPassword.isNullOrBlank()

val scoutKeystoreFile = layout.buildDirectory.file("scout-signing/scout-devnet-signing.jks")

if (scoutSigningAvailable) {
    val keystoreBytes = Base64.getDecoder().decode(scoutKeystoreBase64)
    val keystoreOutput = scoutKeystoreFile.get().asFile

    keystoreOutput.parentFile.mkdirs()
    keystoreOutput.writeBytes(keystoreBytes)
}

android {
    namespace = "com.routalk.scoutoperator"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.routalk.scoutoperator"
        minSdk = 28
        targetSdk = 35
        versionCode = scoutVersionCode
        versionName = scoutVersionName
    }

    signingConfigs {
        if (scoutSigningAvailable) {
            create("scoutDevnet") {
                storeFile = scoutKeystoreFile.get().asFile
                storePassword = scoutStorePassword
                keyAlias = scoutKeyAlias
                keyPassword = scoutKeyPassword
            }
        }
    }

    buildTypes {
        debug {
            if (scoutSigningAvailable) {
                signingConfig = signingConfigs.getByName("scoutDevnet")
            }
        }

        release {
            isMinifyEnabled = false

            if (scoutSigningAvailable) {
                signingConfig = signingConfigs.getByName("scoutDevnet")
            }
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }
}
