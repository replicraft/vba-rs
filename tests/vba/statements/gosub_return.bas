' INV: stmt.gosub-return
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/gosubreturn-statement
' EXPECT: hi
' EXPECT: back
Public Sub Main()
    GoSub Sayhi
    Debug.Print "back"
    Exit Sub
Sayhi:
    Debug.Print "hi"
    Return
End Sub
