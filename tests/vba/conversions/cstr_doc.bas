' INV: func.cstr
' SOURCE: spec ms-vbal 6.1.2.3.1.12 CStr
' EXPECT: 437.324
Public Sub Main()
    Debug.Print CStr(CStr(437.324))
End Sub
