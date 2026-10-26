' INV: type.byte
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT:  255 
' EXPECT:  0 
Public Sub Main()
    Dim b As Byte
    b = 255
    Debug.Print b
    b = 0
    Debug.Print b
End Sub
