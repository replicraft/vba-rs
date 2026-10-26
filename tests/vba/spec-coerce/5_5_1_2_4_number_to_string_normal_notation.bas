' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, number to String normal notation
' EXPECT: zero=0
' EXPECT: dbl=123.45
' EXPECT: neg=-123.45
' EXPECT: long=-7
' EXPECT: trailing=2.5
' EXPECT: whole=100
' EXPECT: half=0.5
' EXPECT: d15=123456789012345
' EXPECT: single7=1234567
' EXPECT: dec21=100000000000000000000
' EXPECT: ll=9223372036854775807
' EXPECT: cur=1234.5
' EXPECT: bool=True,False
Public Sub Main()
    Dim s As String
    Dim d As Double
    Dim sg As Single
    Dim l As Long
    Dim c As Currency
    Dim q As LongLong
    Dim v As Variant
    Dim b As Boolean
    d = 0: s = d: Debug.Print "zero=" & s
    d = 123.45: s = d: Debug.Print "dbl=" & s
    d = -123.45: s = d: Debug.Print "neg=" & s
    l = -7: s = l: Debug.Print "long=" & s
    d = 2.5: s = d: Debug.Print "trailing=" & s
    d = 100: s = d: Debug.Print "whole=" & s
    d = 0.5: s = d: Debug.Print "half=" & s
    d = 123456789012345#: s = d: Debug.Print "d15=" & s
    sg = 1234567: s = sg: Debug.Print "single7=" & s
    v = CDec("100000000000000000000"): s = v: Debug.Print "dec21=" & s
    q = CLngLng("9223372036854775807"): s = q: Debug.Print "ll=" & s
    c = 1234.5: s = c: Debug.Print "cur=" & s
    b = True: s = b
    Dim t As String
    t = s
    b = False: s = b
    Debug.Print "bool=" & t & "," & s
End Sub
