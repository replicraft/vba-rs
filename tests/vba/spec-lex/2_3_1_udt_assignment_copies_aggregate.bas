' INV: vbal.2.3.1
' SOURCE: spec ms-vbal 2.3.1 aggregate variables, assigning a UDT value to another variable copies the aggregate; later element changes do not affect the other variable
' EXPECT: 1 5
' EXPECT: 1 2
Private Type Inner
    V As Long
End Type
Private Type P
    X As Long
    I As Inner
End Type
Public Sub Main()
    Dim a As P, b As P
    a.X = 1
    a.I.V = 2
    b = a
    b.I.V = 5
    Debug.Print b.X & " " & b.I.V
    Debug.Print a.X & " " & a.I.V
End Sub
