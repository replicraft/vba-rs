' INV: type.longlong
' SOURCE: spec ms-vbal 5.6.9.3 arithmetic overflow
' ERROR: 6
Public Sub Main()
    Dim n As LongLong
    n = 9223372036854775807^
    n = n + 1
End Sub
