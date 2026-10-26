' INV: vbal.5.6.9.3.7, op.pow
' SOURCE: spec ms-vbal 5.6.9.3.7 ^ Operator, the effective value type is Double for every operand type pair; 0^0 is 1
' EXPECT: Integer^Integer=Double 8
' EXPECT: Byte^Byte=Double 27
' EXPECT: Long^Integer=Double 1024
' EXPECT: Single^Single=Double 4
' EXPECT: Currency^Integer=Double 25
' EXPECT: String^String=Double 8
' EXPECT: Date^Integer=Double 4
' EXPECT: Boolean^Integer=Double 1
' EXPECT: 0^0=1
' EXPECT: neg exp=0.25
' EXPECT: frac=3
' EXPECT: Empty^Integer=Double 0
' EXPECT: Integer^Empty=Double 1
' EXPECT: 0^2=0
Public Sub Main()
    Dim i As Integer: i = 2
    Dim i3 As Integer: i3 = 3
    Dim y As Byte: y = 3
    Dim l As Long: l = 2
    Dim i10 As Integer: i10 = 10
    Dim s As Single: s = 2
    Dim c As Currency: c = 5
    Dim t As String: t = "2"
    Dim t3 As String: t3 = "3"
    Dim dt As Date: dt = 2
    Dim b As Boolean: b = True
    Dim z As Double: z = 0
    Dim e As Variant
    Dim neg As Integer: neg = -2
    Debug.Print "Integer^Integer=" & TypeName(i ^ i3) & " " & CStr(i ^ i3)
    Debug.Print "Byte^Byte=" & TypeName(y ^ y) & " " & CStr(y ^ y)
    Debug.Print "Long^Integer=" & TypeName(l ^ i10) & " " & CStr(l ^ i10)
    Debug.Print "Single^Single=" & TypeName(s ^ s) & " " & CStr(s ^ s)
    Debug.Print "Currency^Integer=" & TypeName(c ^ i) & " " & CStr(c ^ i)
    Debug.Print "String^String=" & TypeName(t ^ t3) & " " & CStr(t ^ t3)
    Debug.Print "Date^Integer=" & TypeName(dt ^ i) & " " & CStr(dt ^ i)
    Debug.Print "Boolean^Integer=" & TypeName(b ^ i) & " " & CStr(b ^ i)
    Debug.Print "0^0=" & CStr(z ^ z)
    Debug.Print "neg exp=" & CStr(i ^ neg)
    Debug.Print "frac=" & CStr(9 ^ 0.5)
    Debug.Print "Empty^Integer=" & TypeName(e ^ i) & " " & CStr(e ^ i)
    Debug.Print "Integer^Empty=" & TypeName(i ^ e) & " " & CStr(i ^ e)
    Debug.Print "0^2=" & CStr(z ^ i)
End Sub
