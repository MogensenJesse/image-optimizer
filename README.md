# Image Optimizer

![Image Optimizer Screenshot](./screenshot.png) 

Image Optimizer is a high-performance desktop application designed to streamline image compression and optimization without compromising visual quality. It offers advanced tools for rapid batch processing of images, including resizing, format conversion, and compression.

## Features

- **Visual Quality Preservation**: Optimizes file size while maintaining perceptual image quality
- **Format Conversion**: Seamlessly convert between image formats while maintaining visual quality
- **Intelligent Resizing**: Multiple resize modes including width, height, longest and shortest side and aspect ratio preservation
- **Quality Control**: Fine-tune compression levels with format-specific quality settings
- **Batch Processing**: Process multiple images in parallel for maximum efficiency
- **Cross-Platform**: Available for Windows, macOS and Linux

## Use Cases

- **Web Developers**: Reduce page load times by up to 80% with optimized images for better SEO rankings and user experience
- **Content Creators**: Maintain visual quality while reducing file sizes by 30-80% for faster social media uploads
- **Photographers**: Process entire photoshoots in minutes instead of hours while preserving professional quality
- **E-commerce Managers**: Create consistent product images with uniform dimensions and optimal file sizes
- **UI/UX Designers**: Export perfectly sized assets for applications with predictable file sizes
- **Digital Marketers**: Reduce email campaign load times and improve engagement with optimized images
- **Storage Optimization**: Reduce storage requirements for large image collections without sacrificing quality

## Technology Stack

- **Tauri**: Modern framework for building smaller, faster, and more secure desktop applications
- **React**: Component-based UI with hooks for state management
- **SCSS**: Modular styling system with variables
- **Rust**: High-performance, memory-safe language for the core application logic
- **Tokio**: Asynchronous runtime for non-blocking operations
- **libvips**: High-performance image processing via vendored native Rust bindings

## Documentation

For detailed technical information about the architecture, components, and implementation details, please refer to the [Architecture Documentation](./ARCHITECTURE.md).

## Security

- **Unsigned Builds**: Currently, both Windows and macOS builds are not code-signed. This is on the roadmap but doesn't affect the application's security.

- **Security Warnings**: You may encounter security warnings when first running the application:
  - **Windows**: You might see a "Windows protected your PC" message. Click "More info" and then "Run anyway" to proceed.
  - **macOS**: You may need to right-click (or Control-click) the app and select "Open" from the context menu, then click "Open" in the dialog that appears.

- **No Network Requirements**: Image Optimizer processes all images locally without sending any data to external servers.

## Roadmap

- [x] Add SVG support
- [ ] Add expert settings toggle
- [x] Add additional performance optimizations
- [x] Add unsupported image format detection
- [x] Add macOS support
- [x] Add Linux support
- [x] Add updating mechanism
- [ ] Redesign the product website
- [ ] Design product logo & icons
- [ ] Implement code signing for Windows and macOS builds
- [x] Add multi-language support

## License
This project is licensed under the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0). Please review the license terms before using or distributing this software.


-------------------------------------
Translated Report (Full Report Below)
-------------------------------------

Process:               image-optimizer [1534]
Path:                  /Applications/Image Optimizer.app/Contents/MacOS/image-optimizer
Identifier:            com.image-optimizer.app
Version:               0.6.7 (0.6.7)
Code Type:             X86-64 (Native)
Parent Process:        launchd [1]
User ID:               501

Date/Time:             2026-04-15 07:12:59.9269 -0700
OS Version:            macOS 14.7 (23H124)
Report Version:        12
Anonymous UUID:        3A906CE9-59B3-4842-9BF4-C590152041E1


Time Awake Since Boot: 14000 seconds

System Integrity Protection: enabled

Crashed Thread:        0

Exception Type:        EXC_CRASH (SIGABRT)
Exception Codes:       0x0000000000000000, 0x0000000000000000

Termination Reason:    Namespace DYLD, Code 1 Library missing
Library not loaded: /usr/local/opt/vips/lib/libvips.42.dylib
Referenced from: <B204EE5D-725A-3B87-99F9-E94F726023EC> /Applications/Image Optimizer.app/Contents/MacOS/image-optimizer
Reason: tried: '/usr/local/opt/vips/lib/libvips.42.dylib' (no such file), '/System/Volumes/Preboot/Cryptexes/OS/usr/local/opt/vips/lib/libvips.42.dylib' (no such file), '/usr/local/opt/vips/lib/libvips.42.dylib' (no such file)
(terminated at launch; ignore backtrace)

Thread 0 Crashed:
0   dyld                          	    0x7ff819e1987a __abort_with_payload + 10
1   dyld                          	    0x7ff819e327f7 abort_with_payload_wrapper_internal + 82
2   dyld                          	    0x7ff819e32829 abort_with_payload + 9
3   dyld                          	    0x7ff819dbf2b5 dyld4::halt(char const*, dyld4::StructuredError const*) + 335
4   dyld                          	    0x7ff819dbc4bd dyld4::prepare(dyld4::APIs&, dyld3::MachOAnalyzer const*) + 4099
5   dyld                          	    0x7ff819dbb2e4 start + 1812


Thread 0 crashed with X86 Thread State (64-bit):
  rax: 0x0000000002000209  rbx: 0x0000000000000001  rcx: 0x00007ff7b181e518  rdx: 0x00007ff7b181e980
  rdi: 0x0000000000000006  rsi: 0x0000000000000001  rbp: 0x00007ff7b181e560  rsp: 0x00007ff7b181e518
   r8: 0x00007ff7b181e580   r9: 0x0000000000000000  r10: 0x000000000000007e  r11: 0x0000000000000246
  r12: 0x0000000000000000  r13: 0x00007ff7b181e980  r14: 0x0000000000000006  r15: 0x000000000000007e
  rip: 0x00007ff819e1987a  rfl: 0x0000000000000246  cr2: 0x0000000000000000
  
Logical CPU:     0
Error Code:      0x02000209 
Trap Number:     133


Binary Images:
       0x10e6e0000 -        0x10f75ffff com.image-optimizer.app (0.6.7) <b204ee5d-725a-3b87-99f9-e94f726023ec> /Applications/Image Optimizer.app/Contents/MacOS/image-optimizer
    0x7ff819db5000 -     0x7ff819e4581f dyld (*) <3a3cc221-017e-30a8-a2d3-0db1b0e5d805> /usr/lib/dyld
               0x0 - 0xffffffffffffffff ??? (*) <00000000-0000-0000-0000-000000000000> ???

External Modification Summary:
  Calls made by other processes targeting this process:
    task_for_pid: 0
    thread_create: 0
    thread_set_state: 0
  Calls made by this process:
    task_for_pid: 0
    thread_create: 0
    thread_set_state: 0
  Calls made by all processes on this machine:
    task_for_pid: 0
    thread_create: 0
    thread_set_state: 0

VM Region Summary:
ReadOnly portion of Libraries: Total=197.5M resident=0K(0%) swapped_out_or_unallocated=197.5M(100%)
Writable regions: Total=12.3M written=0K(0%) resident=0K(0%) swapped_out=0K(0%) unallocated=12.3M(100%)

                                VIRTUAL   REGION 
REGION TYPE                        SIZE    COUNT (non-coalesced) 
===========                     =======  ======= 
STACK GUARD                       56.0M        1 
Stack                             8192K        1 
VM_ALLOCATE                       4356K        3 
__DATA                             495K        3 
__DATA_CONST                        22K        1 
__DATA_DIRTY                         7K        1 
__LINKEDIT                       180.4M        2 
__TEXT                            17.1M        2 
shared memory                        8K        2 
===========                     =======  ======= 
TOTAL                            266.3M       16 



-----------
Full Report
-----------

{"app_name":"image-optimizer","timestamp":"2026-04-15 07:12:59.00 -0700","app_version":"0.6.7","slice_uuid":"b204ee5d-725a-3b87-99f9-e94f726023ec","build_version":"0.6.7","platform":1,"bundleID":"com.image-optimizer.app","share_with_app_devs":0,"is_first_party":0,"bug_type":"309","os_version":"macOS 14.7 (23H124)","roots_installed":0,"name":"image-optimizer","incident_id":"FBA47C6D-808D-4463-8856-C695E706F8BA"}
{
  "uptime" : 14000,
  "procRole" : "Background",
  "version" : 2,
  "userID" : 501,
  "deployVersion" : 210,
  "modelCode" : "MacBookPro19,1",
  "coalitionID" : 1842,
  "osVersion" : {
    "train" : "macOS 14.7",
    "build" : "23H124",
    "releaseType" : "User"
  },
  "captureTime" : "2026-04-15 07:12:59.9269 -0700",
  "codeSigningMonitor" : 0,
  "incident" : "FBA47C6D-808D-4463-8856-C695E706F8BA",
  "pid" : 1534,
  "cpuType" : "X86-64",
  "roots_installed" : 0,
  "bug_type" : "309",
  "procLaunch" : "2026-04-15 07:12:58.0322 -0700",
  "procStartAbsTime" : 14956278642256,
  "procExitAbsTime" : 14958173008496,
  "procName" : "image-optimizer",
  "procPath" : "\/Applications\/Image Optimizer.app\/Contents\/MacOS\/image-optimizer",
  "bundleInfo" : {"CFBundleShortVersionString":"0.6.7","CFBundleVersion":"0.6.7","CFBundleIdentifier":"com.image-optimizer.app"},
  "storeInfo" : {"deviceIdentifierForVendor":"EB567AA2-F8CB-54F4-8E68-4E56872F53E2","thirdParty":true},
  "parentProc" : "launchd",
  "parentPid" : 1,
  "coalitionName" : "com.image-optimizer.app",
  "crashReporterKey" : "3A906CE9-59B3-4842-9BF4-C590152041E1",
  "codeSigningID" : "",
  "codeSigningTeamID" : "",
  "codeSigningValidationCategory" : 0,
  "codeSigningTrustLevel" : 4294967295,
  "sip" : "enabled",
  "exception" : {"codes":"0x0000000000000000, 0x0000000000000000","rawCodes":[0,0],"type":"EXC_CRASH","signal":"SIGABRT"},
  "termination" : {"code":1,"flags":518,"namespace":"DYLD","indicator":"Library missing","details":["(terminated at launch; ignore backtrace)"],"reasons":["Library not loaded: \/usr\/local\/opt\/vips\/lib\/libvips.42.dylib","Referenced from: <B204EE5D-725A-3B87-99F9-E94F726023EC> \/Applications\/Image Optimizer.app\/Contents\/MacOS\/image-optimizer","Reason: tried: '\/usr\/local\/opt\/vips\/lib\/libvips.42.dylib' (no such file), '\/System\/Volumes\/Preboot\/Cryptexes\/OS\/usr\/local\/opt\/vips\/lib\/libvips.42.dylib' (no such file), '\/usr\/local\/opt\/vips\/lib\/libvips.42.dylib' (no such file)"]},
  "extMods" : {"caller":{"thread_create":0,"thread_set_state":0,"task_for_pid":0},"system":{"thread_create":0,"thread_set_state":0,"task_for_pid":0},"targeted":{"thread_create":0,"thread_set_state":0,"task_for_pid":0},"warnings":0},
  "faultingThread" : 0,
  "threads" : [{"triggered":true,"id":66622,"threadState":{"r13":{"value":140701811730816},"rax":{"value":33554953},"rflags":{"value":582},"cpu":{"value":0},"r14":{"value":6},"rsi":{"value":1},"r8":{"value":140701811729792},"cr2":{"value":0},"rdx":{"value":140701811730816},"r10":{"value":126},"r9":{"value":0},"r15":{"value":126},"rbx":{"value":1},"trap":{"value":133},"err":{"value":33554953},"r11":{"value":582},"rip":{"value":140703562831994,"matchesCrashFrame":1},"rbp":{"value":140701811729760},"rsp":{"value":140701811729688},"r12":{"value":0},"rcx":{"value":140701811729688},"flavor":"x86_THREAD_STATE","rdi":{"value":6}},"frames":[{"imageOffset":411770,"symbol":"__abort_with_payload","symbolLocation":10,"imageIndex":1},{"imageOffset":514039,"symbol":"abort_with_payload_wrapper_internal","symbolLocation":82,"imageIndex":1},{"imageOffset":514089,"symbol":"abort_with_payload","symbolLocation":9,"imageIndex":1},{"imageOffset":41653,"symbol":"dyld4::halt(char const*, dyld4::StructuredError const*)","symbolLocation":335,"imageIndex":1},{"imageOffset":29885,"symbol":"dyld4::prepare(dyld4::APIs&, dyld3::MachOAnalyzer const*)","symbolLocation":4099,"imageIndex":1},{"imageOffset":25316,"symbol":"start","symbolLocation":1812,"imageIndex":1}]}],
  "usedImages" : [
  {
    "source" : "P",
    "arch" : "x86_64",
    "base" : 4537057280,
    "CFBundleShortVersionString" : "0.6.7",
    "CFBundleIdentifier" : "com.image-optimizer.app",
    "size" : 17301504,
    "uuid" : "b204ee5d-725a-3b87-99f9-e94f726023ec",
    "path" : "\/Applications\/Image Optimizer.app\/Contents\/MacOS\/image-optimizer",
    "name" : "image-optimizer",
    "CFBundleVersion" : "0.6.7"
  },
  {
    "source" : "P",
    "arch" : "x86_64",
    "base" : 140703562420224,
    "size" : 591904,
    "uuid" : "3a3cc221-017e-30a8-a2d3-0db1b0e5d805",
    "path" : "\/usr\/lib\/dyld",
    "name" : "dyld"
  },
  {
    "size" : 0,
    "source" : "A",
    "base" : 0,
    "uuid" : "00000000-0000-0000-0000-000000000000"
  }
],
  "sharedCache" : {
  "base" : 140703561723904,
  "size" : 25769803776,
  "uuid" : "0558adbc-51e6-35a7-9a10-a10a1291df47"
},
  "vmSummary" : "ReadOnly portion of Libraries: Total=197.5M resident=0K(0%) swapped_out_or_unallocated=197.5M(100%)\nWritable regions: Total=12.3M written=0K(0%) resident=0K(0%) swapped_out=0K(0%) unallocated=12.3M(100%)\n\n                                VIRTUAL   REGION \nREGION TYPE                        SIZE    COUNT (non-coalesced) \n===========                     =======  ======= \nSTACK GUARD                       56.0M        1 \nStack                             8192K        1 \nVM_ALLOCATE                       4356K        3 \n__DATA                             495K        3 \n__DATA_CONST                        22K        1 \n__DATA_DIRTY                         7K        1 \n__LINKEDIT                       180.4M        2 \n__TEXT                            17.1M        2 \nshared memory                        8K        2 \n===========                     =======  ======= \nTOTAL                            266.3M       16 \n",
  "legacyInfo" : {
  "threadTriggered" : {

  }
},
  "logWritingSignature" : "afa13149d44e9fb6312de828ccc1b811ae3be54b",
  "trialInfo" : {
  "rollouts" : [
    {
      "rolloutId" : "645197bf528fbf3c3af54105",
      "factorPackIds" : {
        "SIRI_VALUE_INFERENCE_PERVASIVE_ENTITY_RESOLUTION" : "663e65b4a1526e1ca0e288a1"
      },
      "deploymentId" : 240000002
    },
    {
      "rolloutId" : "60f8ddccefea4203d95cbeef",
      "factorPackIds" : {

      },
      "deploymentId" : 240000025
    }
  ],
  "experiments" : [

  ]
}
}

Model: MacBookPro19,1, BootROM VMW201.00V.24866131.B64.2507211911, 8 processors, Unknown, 2,57 GHz, 8 GB, SMC 2.8f0
Graphics: Display, 3 MB
Display: Unknown Display, 1024 x 768 (XGA - eXtended Graphics Array), Main, MirrorOff, Online
Memory Module: RAM slot #0/RAM slot #0, 8 GB, DRAM, 4800 MHz, VMware Virtual RAM, VMW-8192MB
Bluetooth: Version (null), 0 services, 0 devices, 0 incoming serial ports
Network Service: Ethernet, Ethernet, en0
PCI Card: sppci_expresscard_name, Ethernet Controller
PCI Card: sppci_expresscard_name, USB eXtensible Host Controller
Serial ATA Device: VMware Virtual SATA Hard Drive, 85,9 GB
Serial ATA Device: VMware Virtual SATA CDRW Drive, 16,64 GB
USB Device: USB20Bus
USB Device: USB32Bus
USB Device: VMware Virtual USB Hub
USB Device: VMware Virtual USB Hub
USB Device: VMware Virtual USB Keyboard
USB Device: VMware Virtual USB Mouse
Thunderbolt Bus: 
