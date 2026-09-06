; Protocol-only toast activation for the existing kukuri: handler.
; A stub CLSID enables Notification Center persistence; it is not a COM server.
!define KUKURI_TOAST_CLSID "{5669FA60-8780-4392-8875-3CAB30E2F1F8}"

!macro KUKURI_SET_TOAST_CLSID shortcut
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  ${If} ${FileExists} "${shortcut}"
    StrCpy $6 -1
    !insertmacro ComHlpr_CreateInProcInstance ${CLSID_ShellLink} ${IID_IShellLink} r0 ""
    ${If} $0 P<> 0
      System::Call '$0->0(g "${IID_IPersistFile}", *p .r1)i'
      ${If} $1 P<> 0
        System::Call '$1->5(w "${shortcut}", i ${STGM_READWRITE})i.r6'
        ${If} $6 >= 0
          StrCpy $6 -1
          System::Call '$0->0(g "${IID_IPropertyStore}", *p .r2)i'
          ${If} $2 P<> 0
            System::Alloc ${SYSSIZEOF_PROPVARIANT}
            Pop $5
            ${If} $5 P<> 0
              System::Call '*$5(&i2 0)'
              System::Call 'propsys::InitPropVariantFromCLSID(g "${KUKURI_TOAST_CLSID}", p r5)i.r6'
              ${If} $6 >= 0
                System::Call '*(&g16 "{9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3}", &i4 26)p.r4'
                ${If} $4 P<> 0
                  System::Call '$2->6(p r4, p r5)i.r6'
                  ${If} $6 >= 0
                    System::Call '$2->7()i.r6'
                  ${EndIf}
                  System::Free $4
                ${Else}
                  StrCpy $6 -1
                ${EndIf}
              ${EndIf}
              System::Call 'ole32::PropVariantClear(p r5)'
              System::Free $5
            ${EndIf}
            System::Call '$2->2()i'
          ${EndIf}
        ${EndIf}
        ${If} $6 >= 0
          System::Call '$1->6(w "${shortcut}", i 1)i.r6'
        ${EndIf}
        System::Call '$1->2()i'
      ${EndIf}
      System::Call '$0->2()i'
    ${EndIf}
    ${If} $6 < 0
      DetailPrint "Could not register the kukuri notification shortcut."
      SetErrors
    ${EndIf}
  ${EndIf}
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ClearErrors
  !if "${STARTMENUFOLDER}" != ""
    !insertmacro KUKURI_SET_TOAST_CLSID "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
  !else
    !insertmacro KUKURI_SET_TOAST_CLSID "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  !endif
  ${If} ${Errors}
    SetErrorLevel 1
    Abort "Could not register the kukuri notification shortcut."
  ${EndIf}
!macroend
