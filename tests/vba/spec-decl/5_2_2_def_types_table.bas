' INV: vbal.5.2.2
' SOURCE: spec ms-vbal 5.2.2 Def directive to declared type table
' EXPECT: Boolean Byte Integer Long LongLong
' EXPECT: Currency Single Double Date String Empty
DefBool A
DefByte B
DefInt C
DefLng D
DefLngLng E
DefCur F
DefSng G
DefDbl H
DefDate I
DefStr J
DefVar K
Dim aa, bb, cc, dd, ee, ff, gg, hh, ii, jj, kk
Public Sub Main()
    Debug.Print TypeName(aa) & " " & TypeName(bb) & " " & TypeName(cc) & " " & TypeName(dd) & " " & TypeName(ee)
    Debug.Print TypeName(ff) & " " & TypeName(gg) & " " & TypeName(hh) & " " & TypeName(ii) & " " & TypeName(jj) & " " & TypeName(kk)
End Sub
