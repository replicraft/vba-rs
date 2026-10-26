' INV: stmt.goto, lex.label
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/goto-statement
' EXPECT:  3 
Public Sub Main()
    Dim i As Long
Top:
    i = i + 1
    If i < 3 Then GoTo Top
    Debug.Print i
End Sub
