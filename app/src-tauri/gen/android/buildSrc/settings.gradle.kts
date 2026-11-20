// The kotlin-dsl plugin pulls Kotlin artifacts while buildSrc is set up. The
// Gradle plugin portal only mirrors them and sometimes misses one (the 0.17.0
// release build failed on kotlin-gradle-plugins-bom), so Maven Central is
// asked first and the portal stays as the fallback.
pluginManagement {
    repositories {
        mavenCentral()
        gradlePluginPortal()
        google()
    }
}
