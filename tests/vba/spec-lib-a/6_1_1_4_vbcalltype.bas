' INV: vbal.6.1.1.4
' SOURCE: spec ms-vbal 6.1.1.4 VbCallType values
' EXPECT: vbGet=2
' EXPECT: vbLet=4
' EXPECT: vbMethod=1
' EXPECT: vbSet=8
Public Sub Main()
    Debug.Print "vbGet=" & CStr(vbGet)
    Debug.Print "vbLet=" & CStr(vbLet)
    Debug.Print "vbMethod=" & CStr(vbMethod)
    Debug.Print "vbSet=" & CStr(vbSet)
End Sub
