' INV: func.cvar
' SOURCE: derived CVar yields a Variant holding the argument's subtype
' EXPECT: Integer
' EXPECT: String
' EXPECT: Double
Public Sub Main()
    Debug.Print CStr(TypeName(CVar(5)))
    Debug.Print CStr(TypeName(CVar("a")))
    Debug.Print CStr(TypeName(CVar(1.5)))
End Sub
