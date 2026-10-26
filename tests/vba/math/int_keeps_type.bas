' INV: func.int, func.fix
' SOURCE: spec ms-vbal 6.1.2.3.1.18 Int
' EXPECT: Double
' EXPECT: Single
' EXPECT: Integer
Public Sub Main()
    Debug.Print CStr(TypeName(Int(5.5)))
    Debug.Print CStr(TypeName(Fix(CSng(5.5))))
    Debug.Print CStr(TypeName(Int(5)))
End Sub
