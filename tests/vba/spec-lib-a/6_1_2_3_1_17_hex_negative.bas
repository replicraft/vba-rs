' INV: vbal.6.1.2.3.1.17, func.hex
' SOURCE: spec ms-vbal 6.1.2.3.1.17 Hex negative values: 16/32/64-bit two's complement by range
' EXPECT: minus1=FFFF
' EXPECT: minus2=FFFE
' EXPECT: minus32767=8001
' EXPECT: minus32768=FFFF8000
' EXPECT: minus32769=FFFF7FFF
' EXPECT: minLong=80000000
' EXPECT: ll=FFFFFFFF7FFFFFFF
' EXPECT: llLen=16
Public Sub Main()
    Debug.Print "minus1=" & Hex(-1)
    Debug.Print "minus2=" & Hex(-2)
    Debug.Print "minus32767=" & Hex(-32767)
    Debug.Print "minus32768=" & Hex(CLng(-32768))
    Debug.Print "minus32769=" & Hex(-32769)
    Debug.Print "minLong=" & Hex(-2147483647 - 1)
    Debug.Print "ll=" & Hex(CLngLng(-2147483649#))
    Debug.Print "llLen=" & CStr(Len(Hex(CLngLng(-2147483649#))))
End Sub
