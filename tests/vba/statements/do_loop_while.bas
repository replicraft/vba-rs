' INV: stmt.do-loop
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/doloop-statement
' EXPECT:  11 
Public Sub Main()
    Dim i As Long
    i = 10
    Do
        i = i + 1
    Loop While i < 5
    Debug.Print i
End Sub
