' INV: op.concat, op.eq, op.precedence
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/operator-precedence
' EXPECT: True
Public Sub Main()
    Debug.Print CStr("a" & "b" = "ab")
End Sub
