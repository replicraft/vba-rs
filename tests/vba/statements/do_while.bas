' INV: stmt.do-loop
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/doloop-statement
' EXPECT:  3 
Public Sub Main()
    Dim i As Long
    i = 0
    Do While i < 3
        i = i + 1
    Loop
    Debug.Print i
End Sub
