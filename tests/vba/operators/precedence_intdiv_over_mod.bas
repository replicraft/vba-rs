' INV: op.precedence, op.intdiv, op.mod
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/operator-precedence
' EXPECT: 1
' EXPECT: 1
Public Sub Main()
    Debug.Print CStr(7 \ 2 Mod 2)
    Debug.Print CStr(11 Mod 4 \ 2)
End Sub
