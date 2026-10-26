' INV: vbal.6.1.2.6.1.12, func.sln
' SOURCE: spec ms-vbal 6.1.2.6.1.12 SLN straight-line depreciation for a single period
' EXPECT: a=1000
' EXPECT: b=450
' EXPECT: type=Double
Public Sub Main()
    Debug.Print "a=" & CStr(SLN(10000, 1000, 9))
    Debug.Print "b=" & CStr(SLN(1000, 100, 2))
    Debug.Print "type=" & TypeName(SLN(10000, 1000, 9))
End Sub
