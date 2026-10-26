' INV: vbal.5.6.9.3, op.add
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators, effective value types for integral operand pairs
' EXPECT: Byte+Byte=Byte
' EXPECT: Byte+Integer=Integer
' EXPECT: Integer+Byte=Integer
' EXPECT: Bool+Bool=Integer
' EXPECT: Bool+Byte=Integer
' EXPECT: Integer+Integer=Integer
' EXPECT: Integer+Long=Long
' EXPECT: Long+Integer=Long
' EXPECT: Byte+Long=Long
' EXPECT: Long+Long=Long
' EXPECT: LongLong+Integer=LongLong
' EXPECT: Byte+LongLong=LongLong
' EXPECT: LongLong+LongLong=LongLong
' EXPECT: boolsum=-2
' EXPECT: bytesum=200
Public Sub Main()
    Dim y As Byte: y = 100
    Dim i As Integer: i = 1
    Dim l As Long: l = 1
    Dim q As LongLong: q = 1
    Dim b As Boolean: b = True
    Debug.Print "Byte+Byte=" & TypeName(y + y)
    Debug.Print "Byte+Integer=" & TypeName(y + i)
    Debug.Print "Integer+Byte=" & TypeName(i + y)
    Debug.Print "Bool+Bool=" & TypeName(b + b)
    Debug.Print "Bool+Byte=" & TypeName(b + y)
    Debug.Print "Integer+Integer=" & TypeName(i + i)
    Debug.Print "Integer+Long=" & TypeName(i + l)
    Debug.Print "Long+Integer=" & TypeName(l + i)
    Debug.Print "Byte+Long=" & TypeName(y + l)
    Debug.Print "Long+Long=" & TypeName(l + l)
    Debug.Print "LongLong+Integer=" & TypeName(q + i)
    Debug.Print "Byte+LongLong=" & TypeName(y + q)
    Debug.Print "LongLong+LongLong=" & TypeName(q + q)
    Debug.Print "boolsum=" & CStr(b + b)
    Debug.Print "bytesum=" & CStr(y + y)
End Sub
