' INV: vbal.5.6.16.1
' SOURCE: spec ms-vbal 5.6.16.1 constant expressions may invoke Int Fix Abs Sgn Len LenB CBool CByte CCur CDate CDbl CInt CLng CLngLng CLngPtr CSng CStr CVar
' XFAIL: wrong pending
' EXPECT: -3
' EXPECT: -3
' EXPECT: 4
' EXPECT: -1
' EXPECT: 3
' EXPECT: True
' EXPECT: 7
' EXPECT: 5
' EXPECT: 2.5
' EXPECT: 6
' EXPECT: 7
' EXPECT: 8
' EXPECT: 1.5
' EXPECT: 123
' EXPECT: 7
' EXPECT: 7
' EXPECT: 0
' EXPECT: True
Private Const I1 As Long = Int(-2.5)
Private Const I2 As Long = Fix(-3.9)
Private Const I3 As Long = Abs(-4)
Private Const I4 As Long = Sgn(-9)
Private Const L1 As Long = Len("abc")
Private Const L2 As Long = LenB("abc")
Private Const K1 As Boolean = CBool(1)
Private Const K2 As Byte = CByte(7)
Private Const K3 As Integer = CInt(4.6)
Private Const K4 As Double = CDbl("2.5")
Private Const K5 As Long = CLng(5.5)
Private Const K6 As LongLong = CLngLng(7)
Private Const K7 As LongPtr = CLngPtr(8)
Private Const K8 As Single = CSng(1.5)
Private Const K9 As String = CStr(123)
Private Const K10 As Currency = CCur(7)
Private Const K11 As Variant = CVar(7)
Private Const K12 As Date = CDate(0)
Public Sub Main()
    Debug.Print CStr(I1)
    Debug.Print CStr(I2)
    Debug.Print CStr(I3)
    Debug.Print CStr(I4)
    Debug.Print CStr(L1)
    Debug.Print CStr(K1)
    Debug.Print CStr(K2)
    Debug.Print CStr(K3)
    Debug.Print CStr(K4)
    Debug.Print CStr(K5)
    Debug.Print CStr(K6)
    Debug.Print CStr(K7)
    Debug.Print CStr(K8)
    Debug.Print K9
    Debug.Print CStr(K10)
    Debug.Print CStr(K11)
    Debug.Print CStr(CDbl(K12))
    Debug.Print CStr(L2 > 0)
End Sub
