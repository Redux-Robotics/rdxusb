## rdxusb - a cross-platform USB library for Redux devices

This library provides an easy to build and easy to link API.
It automatically handles connecting to compatible Redux devices.

## Installation - Rust

```bash
cargo add rdxusb
```

## Installation - Maven

RdxUsb builds for every WPILib-supported platform.

If you're using nativeutils, consider the following modifications to your gradle build:

```groovy
// Add the Redux Maven url in your repositories block:
repositories {
  // ...
  maven {
    url = "https://maven.reduxrobotics.com/"
  }
}

nativeUtils {
  nativeDependencyContainer {
    // this can be static or dynamic, doesn't really matter
    rdxusb(getNativeDependencyTypeClass('WPIStaticMavenDependency')) {
      version = "2025.+"
      groupId = "com.reduxrobotics.usb"
      artifactId = "rdxusb"
      ext = "zip"

      headerClassifier = "headers"
      targetPlatforms = [
        "windowsx86-64",
        "windowsarm64",
        "linuxarm64",
        "linuxx86-64",
        "linuxathena",
        "linuxarm32",
        "osxuniversal"
      ]
    }
  }
}

model {
  components {
    YourLibrary(NativeLibrarySpec) {
      binaries.all {        
        if (it.targetPlatform.operatingSystem.isMacOsX()) {
          // MacOS needs to be linked against IOKit
          it.linker.args << '-framework' << 'IOKit'
        }
      }
      nativeUtils.useRequiredLibrary(it, "rdxusb", [...your stuff here...])
    }
  }
}
```

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
