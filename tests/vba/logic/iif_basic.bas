' INV: func.iif
' SOURCE: spec ms-vbal 6.1.2.8.1.11 IIf
' EXPECT: 10
' EXPECT: b
Public Sub Main()
    Debug.Print CStr(IIf(1 < 2, 10, 20))
    Debug.Print CStr(IIf(False, "a", "b"))
End Sub
