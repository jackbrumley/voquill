!macro NSIS_HOOK_POSTINSTALL
  CopyFiles /SILENT "$INSTDIR\packaging\windows\vulkan-1.dll" "$INSTDIR\vulkan-1.dll"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  Delete "$INSTDIR\vulkan-1.dll"
!macroend
