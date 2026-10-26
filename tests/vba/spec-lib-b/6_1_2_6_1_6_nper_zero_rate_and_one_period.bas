' INV: vbal.6.1.2.6.1.6, func.nper
' SOURCE: spec ms-vbal 6.1.2.6.1.6 NPer sign convention, optional FV and Due omitted
' EXPECT: zero=10
' EXPECT: one=1
' EXPECT: type=Double
Public Sub Main()
    Debug.Print "zero=" & CStr(Round(NPer(0, -100, 1000), 6))
    Debug.Print "one=" & CStr(Round(NPer(0.1, -110, 100), 6))
    Debug.Print "type=" & TypeName(NPer(0, -100, 1000))
End Sub
