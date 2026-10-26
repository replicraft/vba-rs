' INV: vbal.6.1.2.7.1.10, func.qbcolor
' SOURCE: spec ms-vbal 6.1.2.7.1.10 QBColor outside 0-15 raises Error 5 (negative)
' ERROR: 5
Public Sub Main()
    Debug.Print "bad=" & CStr(QBColor(-1))
End Sub
