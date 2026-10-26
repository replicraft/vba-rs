' INV: vbal.5.5.1.2.9, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.9 Let-coercion to and from Error, a value between 0 and 65535 after Long coercion converts to an Error value
' EXPECT: 0 iserror=True
' EXPECT: 65535 iserror=True
' EXPECT: 2.5 iserror=True
' EXPECT: string iserror=True
Public Sub Main()
    Debug.Print "0 iserror=" & CStr(IsError(CVErr(0)))
    Debug.Print "65535 iserror=" & CStr(IsError(CVErr(65535)))
    Debug.Print "2.5 iserror=" & CStr(IsError(CVErr(2.5)))
    Debug.Print "string iserror=" & CStr(IsError(CVErr("13")))
End Sub
