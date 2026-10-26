' INV: func.dateserial
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/dateserial-function
' EXPECT:  1969  2  12 
' EXPECT:  1980  5  31 
Public Sub Main()
    Dim d As Date
    d = DateSerial(1969, 2, 12)
    Debug.Print Year(d); Month(d); Day(d)
    d = DateSerial(1990 - 10, 8 - 2, 1 - 1)
    Debug.Print Year(d); Month(d); Day(d)
End Sub
