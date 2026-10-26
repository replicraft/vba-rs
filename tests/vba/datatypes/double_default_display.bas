' INV: type.double
' SOURCE: unverified current behavior of the implementation; the exact switch to exponent notation was not checked against a document
' EXPECT:  1E+15 
' EXPECT:  100000000000000 
' EXPECT:  0.0001 
' EXPECT:  1E-05 
' EXPECT:  1234567.891 
Public Sub Main()
    Debug.Print 1E15
    Debug.Print 1E14
    Debug.Print 0.0001
    Debug.Print 0.00001
    Debug.Print 1234567.891
End Sub
