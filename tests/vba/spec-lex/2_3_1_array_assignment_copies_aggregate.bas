' INV: vbal.2.3.1
' SOURCE: spec ms-vbal 2.3.1 aggregate variables, an aggregate value is immutable; changing an element of one variable leaves other variables holding the old value
' EXPECT: 1 9
' EXPECT: 1 2
Public Sub Main()
    Dim a As Variant
    Dim b As Variant
    a = Array(1, 2)
    b = a
    b(1) = 9
    Debug.Print a(0) & " " & b(1)
    Dim c(0 To 1) As Long
    Dim d() As Long
    c(0) = 1: c(1) = 2
    d = c
    d(1) = 7
    Debug.Print c(0) & " " & c(1)
End Sub
