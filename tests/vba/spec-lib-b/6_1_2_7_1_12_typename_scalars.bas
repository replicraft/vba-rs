' INV: vbal.6.1.2.7.1.12, func.typename
' SOURCE: spec ms-vbal 6.1.2.7.1.12 TypeName strings for scalar value types
' EXPECT: Byte
' EXPECT: Integer
' EXPECT: Long
' EXPECT: LongLong
' EXPECT: Single
' EXPECT: Double
' EXPECT: Currency
' EXPECT: Decimal
' EXPECT: Date
' EXPECT: String
' EXPECT: Boolean
' EXPECT: Empty
' EXPECT: Null
' EXPECT: Error
Public Sub Main()
    Debug.Print TypeName(CByte(1))
    Debug.Print TypeName(CInt(1))
    Debug.Print TypeName(CLng(1))
    Debug.Print TypeName(CLngLng(1))
    Debug.Print TypeName(CSng(1))
    Debug.Print TypeName(CDbl(1))
    Debug.Print TypeName(CCur(1))
    Debug.Print TypeName(CDec(1))
    Debug.Print TypeName(DateSerial(2020, 1, 2))
    Debug.Print TypeName("s")
    Debug.Print TypeName(True)
    Debug.Print TypeName(Empty)
    Debug.Print TypeName(Null)
    Debug.Print TypeName(CVErr(5))
End Sub
