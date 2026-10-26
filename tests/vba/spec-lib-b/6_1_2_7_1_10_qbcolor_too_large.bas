' INV: vbal.6.1.2.7.1.10, func.qbcolor
' SOURCE: spec ms-vbal 6.1.2.7.1.10 QBColor outside 0-15 raises Error 5
' EXPECT: ok=15
' ERROR: 5
Public Sub Main()
    Debug.Print "ok=" & CStr(QBColor(15) And 15)
    Debug.Print "bad=" & CStr(QBColor(16))
End Sub
