' INV: stmt.do-loop, stmt.exit
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/exit-statement
' EXPECT:  5 
Public Sub Main()
    Dim i As Long
    Do
        i = i + 1
        If i = 5 Then Exit Do
    Loop
    Debug.Print i
End Sub
