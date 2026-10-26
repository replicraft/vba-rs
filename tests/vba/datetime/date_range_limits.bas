' INV: type.date
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT:  100 
' EXPECT:  9999  12  31 
' EXPECT:  2958465 
Public Sub Main()
    Dim d As Date
    d = DateSerial(100, 1, 1)
    Debug.Print Year(d)
    d = DateSerial(9999, 12, 31)
    Debug.Print Year(d); Month(d); Day(d)
    Debug.Print CDbl(d)
End Sub
