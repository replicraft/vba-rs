' INV: func.int, func.fix
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/int-fix-functions
' EXPECT: True
' EXPECT: -9
' EXPECT: -8
Public Sub Main()
    Debug.Print CStr(Fix(-8.4) = Sgn(-8.4) * Int(Abs(-8.4)))
    Debug.Print CStr(Int(-8.4))
    Debug.Print CStr(Fix(-8.4))
End Sub
