' INV: stmt.while-wend
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/whilewend-statement
' EXPECT:  3 
Public Sub Main()
    Dim i As Long
    While i < 3
        i = i + 1
    Wend
    Debug.Print i
End Sub
