' INV: func.cdate
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/type-conversion-functions
' EXPECT:  1969  2  12 
Public Sub Main()
    Dim d As Date
    d = CDate("February 12, 1969")
    Debug.Print Year(d); Month(d); Day(d)
End Sub
