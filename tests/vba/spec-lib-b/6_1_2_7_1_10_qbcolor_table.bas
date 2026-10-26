' INV: vbal.6.1.2.7.1.10, func.qbcolor
' SOURCE: spec ms-vbal 6.1.2.7.1.10 QBColor table of returned values
' EXPECT: 0=0
' EXPECT: 1=8388608
' EXPECT: 2=32768
' EXPECT: 3=8421376
' EXPECT: 4=128
' EXPECT: 5=8388736
' EXPECT: 6=32896
' EXPECT: 7=12632256
' EXPECT: 8=8421504
' EXPECT: 9=16711680
' EXPECT: 10=65280
' EXPECT: 11=16776960
' EXPECT: 12=255
' EXPECT: 13=16711935
' EXPECT: 14=65535
' EXPECT: 15=16777215
Public Sub Main()
    Dim i As Integer
    For i = 0 To 15
        Debug.Print CStr(i) & "=" & CStr(QBColor(i))
    Next i
End Sub
