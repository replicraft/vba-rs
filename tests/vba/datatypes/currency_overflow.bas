' INV: type.currency
' SOURCE: spec ms-vbal 5.6.9.3 arithmetic overflow
' ERROR: 6
Public Sub Main()
    Dim c As Currency
    c = CCur("922337203685477.5807")
    c = c + 1
End Sub
