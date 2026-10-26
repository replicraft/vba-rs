' INV: vbal.2.1, type.longlong
' SOURCE: spec ms-vbal 2.1 data values and value types, LongLong holds the full signed 64-bit range
' EXPECT: 9223372036854775807
' EXPECT: -9223372036854775808
' EXPECT: LongLong
Public Sub Main()
    Dim hi As LongLong, lo As LongLong
    hi = 9223372036854775807^
    lo = -9223372036854775807^ - 1
    Debug.Print CStr(hi)
    Debug.Print CStr(lo)
    Debug.Print TypeName(hi)
End Sub
