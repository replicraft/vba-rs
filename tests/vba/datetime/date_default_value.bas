' INV: type.date
' SOURCE: spec ms-vbal 2.3 variable default value for Date and 5.5.1.2.3 Date to Double
' EXPECT:  0 
' EXPECT:  1899 
Public Sub Main()
    Dim d As Date
    Debug.Print CDbl(d)
    Debug.Print Year(d)
End Sub
