' INV: type.currency, op.add
' SOURCE: spec ms-vbal 5.6.9.3 arithmetic operators (Currency)
' EXPECT: True
' EXPECT:  0.9 
Public Sub Main()
    Dim c As Currency
    c = 0.1
    c = c + 0.2
    Debug.Print c = 0.3
    Debug.Print c * 3
End Sub
