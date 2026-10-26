' INV: type.single
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT:  0.1 
' EXPECT: False
' EXPECT: False
Public Sub Main()
    Dim s As Single
    s = 0.1
    Debug.Print s
    Debug.Print s = 0.1
    Debug.Print CDbl(s) = 0.1
End Sub
