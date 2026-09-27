plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    // `in.etamil` is what reversing the domain would give, and it is not a legal
    // package name: `in` is a Java keyword, so no source file could declare it.
    // Hence `org.etamil.mobile`. The JNI symbol names in android/rust/src/lib.rs
    // are derived from this — `Java_org_etamil_mobile_Etamil_nativeRun` — so
    // changing it here without changing them there produces an app that
    // installs, launches and then dies on `UnsatisfiedLinkError`.
    namespace = "org.etamil.mobile"
    compileSdk = 35

    defaultConfig {
        applicationId = "org.etamil.mobile"
        // Android 7.0. The Rust standard library supports rather older than
        // this, but 24 is where `File`-based APIs and the current AndroidX
        // baseline stop needing workarounds, and it still reaches essentially
        // every device in use.
        minSdk = 24
        targetSdk = 35
        versionCode = 3
        // Kept in step with the compiler's own version by hand. There is only
        // one place to look — etamil_compiler/Cargo.toml — and the app reports
        // the compiler's version from the library itself, so a drift here is
        // cosmetic rather than misleading.
        versionName = "1.2.0"
    }

    // No `abiFilters`. Whatever cargo-ndk has put in src/main/jniLibs is what
    // the APK carries, which means the CI job's ABI list is the single place
    // that decides — rather than here and there, disagreeing.
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }

    signingConfigs {
        create("release") {
            // Supplied by CI from repository secrets, absent everywhere else.
            // A developer running `assembleRelease` locally gets the debug key
            // instead of a build failure — see the buildTypes block.
            val storePath = System.getenv("ETAMIL_KEYSTORE_PATH")
            if (storePath != null && file(storePath).exists()) {
                storeFile = file(storePath)
                storePassword = System.getenv("ETAMIL_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("ETAMIL_KEY_ALIAS")
                keyPassword = System.getenv("ETAMIL_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        debug {
            // So a debug and a release build can sit on one phone at once,
            // which is what you want the first time a release build misbehaves.
            applicationIdSuffix = ".debug"
        }
        release {
            // The Rust library is already stripped and built at opt-level "z";
            // shrinking the few hundred lines of Kotlin on top of it would save
            // nothing worth the risk of R8 removing a JNI entry point.
            isMinifyEnabled = false
            signingConfig = if (System.getenv("ETAMIL_KEYSTORE_PATH") != null) {
                signingConfigs.getByName("release")
            } else {
                // An unsigned release APK cannot be installed at all, so an
                // absent keystore would turn a working build into a useless
                // artifact. Debug-signed is honest about what it is: installable
                // for testing, not publishable.
                signingConfigs.getByName("debug")
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

    packaging {
        // The .so files are already stripped by the release profile in
        // android/rust/Cargo.toml. Letting Gradle also try produces a warning
        // about a missing strip tool on any machine whose NDK layout it does
        // not recognise, and changes nothing.
        jniLibs {
            keepDebugSymbols += "**/*.so"
        }
    }
}

dependencies {
    // Deliberately short. This app is a text box, a button and a scrolling
    // output pane over a Rust library; anything more would be dependencies
    // carried for their own sake.
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("com.google.android.material:material:1.12.0")
}
