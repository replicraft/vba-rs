' INV: stmt.let
' SOURCE: spec ms-vbal 5.4.3.8 Let statement
' EXPECT:  4 
Public Sub Main()
    Dim x As Long
    Let x = 4
    Debug.Print x
End Sub
