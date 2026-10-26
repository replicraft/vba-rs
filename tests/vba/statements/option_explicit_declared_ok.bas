' INV: stmt.option-explicit
' SOURCE: spec ms-vbal 5.2.1.3 Option Explicit directive
' EXPECT:  1 
Option Explicit
Public Sub Main()
    Dim x As Long
    x = 1
    Debug.Print x
End Sub
