' INV: stmt.if-then-else
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/ifthenelse-statement
' EXPECT: big
' EXPECT: small
Public Sub Main()
    Dim x As Long
    x = 5
    If x > 3 Then Debug.Print "big" Else Debug.Print "small"
    If x > 9 Then Debug.Print "big" Else Debug.Print "small"
End Sub
