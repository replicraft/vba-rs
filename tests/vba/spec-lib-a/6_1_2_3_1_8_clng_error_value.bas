' INV: vbal.6.1.2.3.1.8, func.clng
' SOURCE: spec ms-vbal 6.1.2.3.1.8 CLng of an Error value returns its error code
' EXPECT: err5=5
' EXPECT: err65535=65535
Public Sub Main()
    Debug.Print "err5=" & CStr(CLng(CVErr(5)))
    Debug.Print "err65535=" & CStr(CLng(CVErr(65535)))
End Sub
