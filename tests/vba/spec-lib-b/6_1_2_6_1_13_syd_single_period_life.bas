' INV: vbal.6.1.2.6.1.13, func.syd
' SOURCE: spec ms-vbal 6.1.2.6.1.13 SYD with Life 1 depreciates the whole depreciable amount in period 1
' EXPECT: whole=900
' EXPECT: type=Double
Public Sub Main()
    Debug.Print "whole=" & CStr(Round(SYD(1000, 100, 1, 1), 6))
    Debug.Print "type=" & TypeName(SYD(1000, 100, 1, 1))
End Sub
