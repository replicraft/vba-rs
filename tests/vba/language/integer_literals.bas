' INV: lit.integer
' SOURCE: spec ms-vbal 3.3.2 number tokens (with 6.1.2.7.1.12 TypeName)
' EXPECT: IntegerIntegerLongLongDouble
Public Sub Main()
    Debug.Print TypeName(1); TypeName(32767); TypeName(32768); TypeName(2147483647); TypeName(2147483648#)
End Sub
