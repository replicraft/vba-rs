' INV: stmt.while-wend, stmt.goto
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/whilewend-statement
' EXPECT:  4 
Public Sub Main()
    Dim i As Long
    While True
        i = i + 1
        If i > 3 Then GoTo Out
    Wend
Out:
    Debug.Print i
End Sub
