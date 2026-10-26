' INV: func.cvar
' SOURCE: spec ms-vbal 6.1.2.3.1.13 CVar
' EXPECT: Integer
' EXPECT: String
' EXPECT: Double
Public Sub Main()
    Debug.Print CStr(TypeName(CVar(5)))
    Debug.Print CStr(TypeName(CVar("a")))
    Debug.Print CStr(TypeName(CVar(1.5)))
End Sub
