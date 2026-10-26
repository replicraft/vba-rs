' INV: type.integer
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT:  32767 
' EXPECT: -32768 
Public Sub Main()
    Dim i As Integer
    i = 32767
    Debug.Print i
    i = -32768
    Debug.Print i
End Sub
