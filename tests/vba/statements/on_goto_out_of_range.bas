' INV: stmt.on-gosub-on-goto
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/ongosub-ongoto-statements
' EXPECT: fell
Public Sub Main()
    On 7 GoTo A, B
    Debug.Print "fell"
    Exit Sub
A:
B:
    Debug.Print "no"
End Sub
