import java.util.Base64

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

val scoutKeystoreBase64 = System.getenv("SCOUT_SIGNING_KEYSTORE_B64")
val scoutStorePassword = System.getenv("SCOUT_SIGNING_STORE_PASSWORD")
val scoutKeyAlias = System.getenv("SCOUT_SIGNING_KEY_ALIAS")
val scoutKeyPassword = System.getenv("SCOUT_SIGNING_KEY_PASSWORD")

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
        versionCode = 2
        versionName = "0.2.0"
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
