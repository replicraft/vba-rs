' INV: stmt.gosub-return
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/gosubreturn-statement
' EXPECT: a1
' EXPECT: b
' EXPECT: a2
Public Sub Main()
    GoSub A
    Exit Sub
A:
    Debug.Print "a1"
    GoSub B
    Debug.Print "a2"
    Return
B:
    Debug.Print "b"
    Return
End Sub
