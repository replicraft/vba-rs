' INV: stmt.do-loop
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/doloop-statement
' EXPECT:  4 
Public Sub Main()
    Dim i As Long
    Do Until i >= 4
        i = i + 2
    Loop
    Debug.Print i
End Sub
