' INV: vbal.6.1.1.15
' SOURCE: spec ms-vbal 6.1.1.15 VbTriState values
' EXPECT: vbFalse=0
' EXPECT: vbTrue=-1
' EXPECT: vbUseDefault=-2
Public Sub Main()
    Debug.Print "vbFalse=" & CStr(vbFalse)
    Debug.Print "vbTrue=" & CStr(vbTrue)
    Debug.Print "vbUseDefault=" & CStr(vbUseDefault)
End Sub
