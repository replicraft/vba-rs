' INV: op.concat, op.add, op.precedence
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/operator-precedence
' EXPECT: 33
' EXPECT: a3
Public Sub Main()
    Debug.Print CStr(1 + 2 & 3)
    Debug.Print CStr("a" & 1 + 2)
End Sub
