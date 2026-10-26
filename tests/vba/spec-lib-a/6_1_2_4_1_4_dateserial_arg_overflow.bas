' INV: vbal.6.1.2.4.1.4, func.dateserial
' SOURCE: spec ms-vbal 6.1.2.4.1.4 DateSerial arguments are Integer: out of range value overflows (error 6)
' EXPECT: month40000=6
' EXPECT: day40000=6
' EXPECT: year40000=6
' EXPECT: boundary=0
Public Sub Main()
    Dim v As Variant
    On Error Resume Next
    Err.Clear: v = DateSerial(2020, 40000, 1): Debug.Print "month40000=" & Err.Number
    Err.Clear: v = DateSerial(2020, 1, 40000): Debug.Print "day40000=" & Err.Number
    Err.Clear: v = DateSerial(40000, 1, 1): Debug.Print "year40000=" & Err.Number
    Err.Clear: v = DateSerial(2020, 1, 32767): Debug.Print "boundary=" & Err.Number
End Sub
