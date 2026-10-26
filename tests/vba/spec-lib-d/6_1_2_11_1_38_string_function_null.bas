' INV: vbal.6.1.2.11.1.38, func.string
' SOURCE: spec ms-vbal 6.1.2.11.1.38 String returns Null when Number or Character is Null
' XFAIL: wrong pending
' EXPECT: e=True
' EXPECT: f=True
Public Sub Main()
    Debug.Print "e=" & CStr(IsNull(String(Null, "a")))
    Debug.Print "f=" & CStr(IsNull(String(3, Null)))
End Sub
