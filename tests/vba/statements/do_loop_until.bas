' INV: stmt.do-loop
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/doloop-statement
' EXPECT:  3 
Public Sub Main()
    Dim i As Long
    Do
        i = i + 1
    Loop Until i = 3
    Debug.Print i
End Sub
