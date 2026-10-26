' INV: stmt.on-gosub-on-goto, stmt.gosub-return
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/ongosub-ongoto-statements
' EXPECT: b
Public Sub Main()
    On 2 GoSub A, B
    Exit Sub
A:
    Debug.Print "a"
    Return
B:
    Debug.Print "b"
    Return
End Sub
