' INV: type.double, lit.float
' SOURCE: spec ms-vbal 3.3.2 Number Tokens (FLOAT)
' EXPECT:  1500 
' EXPECT:  0.0025 
' EXPECT:  100 
' EXPECT:  123456789012345 
Public Sub Main()
    Debug.Print 1.5E3
    Debug.Print 2.5E-3
    Debug.Print 1E+2
    Debug.Print 123456789012345#
End Sub
