' INV: vbal.6.1.1.13
' SOURCE: spec ms-vbal 6.1.1.13 VbQueryClose values
' EXPECT: vbAppTaskManager=3
' EXPECT: vbAppWindows=2
' EXPECT: vbFormCode=1
' EXPECT: vbFormControlMenu=0
' EXPECT: vbFormMDIForm=4
Public Sub Main()
    Debug.Print "vbAppTaskManager=" & CStr(vbAppTaskManager)
    Debug.Print "vbAppWindows=" & CStr(vbAppWindows)
    Debug.Print "vbFormCode=" & CStr(vbFormCode)
    Debug.Print "vbFormControlMenu=" & CStr(vbFormControlMenu)
    Debug.Print "vbFormMDIForm=" & CStr(vbFormMDIForm)
End Sub
