; src-tauri/windows/hooks.nsh
;
; NSIS installer hooks for bundling native libvips DLLs.
; Bundled DLLs land in $INSTDIR\native-dlls\ via Tauri's
; bundle.resources config. The POSTINSTALL hook copies them next to
; the executable where the Windows DLL loader can find them.

!macro NSIS_HOOK_POSTINSTALL
  ${If} ${FileExists} "$INSTDIR\native-dlls\*.dll"
    CopyFiles /SILENT "$INSTDIR\native-dlls\*.dll" "$INSTDIR\"
    RMDir /r "$INSTDIR\native-dlls"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Remove DLLs that POSTINSTALL copied next to the executable
  Delete "$INSTDIR\lib*.dll"
!macroend
