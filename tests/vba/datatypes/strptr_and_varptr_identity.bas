' INV: func.strptr, func.varptr
' SOURCE: derived StrPtr of vbNullString is 0 and of other text nonzero; VarPtr names a variable, the same one every time and, through a ByRef parameter, the caller's variable; real VBA returns addresses, this implementation opaque tokens
' METHOD: by hand; the token values are never printed
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
Private Sub Check(ByRef x As Long, ByVal expected As Variant)
    Debug.Print VarPtr(x) = expected
End Sub
Public Sub Main()
    Dim v1 As Long
    Dim v2 As Long
    Debug.Print StrPtr(vbNullString) = 0
    Debug.Print StrPtr("abc") <> 0
    Debug.Print VarPtr(v1) = VarPtr(v1)
    Debug.Print VarPtr(v1) <> VarPtr(v2)
    Check v1, VarPtr(v1)
End Sub
