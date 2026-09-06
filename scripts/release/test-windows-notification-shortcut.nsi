; File-only contract test. It never installs an app or changes Start Menu/registry.
Unicode true
SilentInstall silent
RequestExecutionLevel user
!include LogicLib.nsh
!include FileFunc.nsh
!include "Win\COM.nsh"
!include "Win\Propkey.nsh"
!include "${HOOK_PATH}"
OutFile "${TEST_OUTPUT}\shortcut-contract.exe"

Function ReadToastClsid
  StrCpy $7 "<error>"
  !insertmacro ComHlpr_CreateInProcInstance ${CLSID_ShellLink} ${IID_IShellLink} r0 ""
  ${If} $0 P<> 0
    System::Call '$0->0(g "${IID_IPersistFile}", *p .r1)i'
    ${If} $1 P<> 0
      System::Call '$1->5(w "$EXEDIR\notification.lnk", i ${STGM_READ})i.r6'
      ${If} $6 >= 0
        System::Call '$0->0(g "${IID_IPropertyStore}", *p .r2)i'
        ${If} $2 P<> 0
          System::Call '*(&g16 "{9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3}", &i4 26)p.r4'
          System::Alloc ${SYSSIZEOF_PROPVARIANT}
          Pop $5
          System::Call '*$5(&i2 0)'
          System::Call '$2->5(p r4, p r5)i.r6'
          FileWrite $9 "GetValue: $6$\r$\n"
          ${If} $6 >= 0
            System::Call '*$5(&i2 .r8, &i6, p .r3)'
            FileWrite $9 "read variant type: $8$\r$\n"
            ${If} $8 = ${VT_EMPTY}
              StrCpy $7 "<absent>"
            ${ElseIf} $8 = ${VT_CLSID}
              System::Call 'ole32::StringFromGUID2(p r3, w .r7, i ${NSIS_MAX_STRLEN})'
            ${EndIf}
          ${EndIf}
          System::Call 'ole32::PropVariantClear(p r5)'
          System::Free $4
          System::Free $5
          System::Call '$2->2()i'
        ${EndIf}
      ${EndIf}
      System::Call '$1->2()i'
    ${EndIf}
    System::Call '$0->2()i'
  ${EndIf}
FunctionEnd

Section
  System::Call 'ole32::CoInitializeEx(p 0, i 2)i.r6'
  ${If} $6 < 0
    SetErrorLevel 19
    Quit
  ${EndIf}
  FileOpen $9 "$EXEDIR\trace.txt" w
  FileWrite $9 "ptr=${NSIS_PTR_SIZE} variant=${SYSSIZEOF_PROPVARIANT}$\r$\n"
  FileWrite $9 "create shortcut$\r$\n"
  CreateShortcut "$EXEDIR\notification.lnk" "$EXEPATH"
  FileWrite $9 "read baseline$\r$\n"
  Call ReadToastClsid
  FileWrite $9 "baseline: $7$\r$\n"
  Call ReadToastClsid
  FileWrite $9 "baseline reread: $7$\r$\n"
  ${If} $7 != "<absent>"
    SetErrorLevel 20
    Quit
  ${EndIf}
  ClearErrors
  StrCpy $0 "preserve-registers"
  !insertmacro KUKURI_SET_TOAST_CLSID "$EXEDIR\notification.lnk"
  FileWrite $9 "hook returned$\r$\n"
  ${If} ${Errors}
    SetErrorLevel 21
    Quit
  ${EndIf}
  ${If} $0 != "preserve-registers"
    SetErrorLevel 22
    Quit
  ${EndIf}
  Call ReadToastClsid
  FileWrite $9 "after hook: $7$\r$\n"
  ${If} $7 != "${KUKURI_TOAST_CLSID}"
    SetErrorLevel 23
    Quit
  ${EndIf}
  ; Reapplying to an existing shortcut must preserve the same typed GUID.
  FileWrite $9 "second hook begin$\r$\n"
  !insertmacro KUKURI_SET_TOAST_CLSID "$EXEDIR\notification.lnk"
  FileWrite $9 "second hook end$\r$\n"
  Call ReadToastClsid
  FileWrite $9 "second read: $7$\r$\n"
  ${If} $7 != "${KUKURI_TOAST_CLSID}"
    SetErrorLevel 24
    Quit
  ${EndIf}
  ; Explicit no-shortcut installs must not create a shortcut through the hook.
  ClearErrors
  FileWrite $9 "missing hook begin$\r$\n"
  !insertmacro KUKURI_SET_TOAST_CLSID "$EXEDIR\not-created.lnk"
  FileWrite $9 "missing hook end$\r$\n"
  ${If} ${Errors}
    SetErrorLevel 25
    Quit
  ${EndIf}
  FileOpen $8 "$EXEDIR\invalid.lnk" w
  FileWrite $8 "not a shortcut"
  FileClose $8
  ClearErrors
  FileWrite $9 "invalid hook begin$\r$\n"
  !insertmacro KUKURI_SET_TOAST_CLSID "$EXEDIR\invalid.lnk"
  FileWrite $9 "invalid hook end$\r$\n"
  ${IfNot} ${Errors}
    SetErrorLevel 26
    Quit
  ${EndIf}
  SetErrorLevel 0
  FileClose $9
  System::Call 'ole32::CoUninitialize()'
SectionEnd
